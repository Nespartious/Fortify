#!/usr/bin/env python3
"""
Onion Proxy - TCP proxy through Tor with circuit renewal and health monitoring.

Features:
- Periodic circuit refresh to prevent stale connections
- Connection health checks with keepalive
- Automatic reconnection on failure
- Connection pooling with staleness detection
- NEWNYM signaling for automatic circuit recovery
"""

import socket
import select
import sys
import struct
import argparse
import logging
import time
import threading
import os
from collections import defaultdict

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Configuration constants
CONNECTION_TIMEOUT = 15  # Timeout for SOCKS connection
IDLE_TIMEOUT = 300  # 5 minutes - close idle connections
HEALTH_CHECK_INTERVAL = 60  # Check connection health every minute
CIRCUIT_REFRESH_INTERVAL = 600  # Refresh circuits every 10 minutes
MAX_CONNECTION_AGE = 1800  # Maximum connection age before forced refresh (30 min)
MAX_CONSECUTIVE_FAILURES = 3  # Failures before circuit refresh
NEWNYM_COOLDOWN = 10  # Minimum seconds between NEWNYM signals


class TorController:
    """Interface to Tor control port for circuit management."""
    
    def __init__(self, control_host='127.0.0.1', control_port=9151, cookie_path=None):
        self.control_host = control_host
        self.control_port = control_port
        self.cookie_path = cookie_path
        self.last_newnym = 0
        self._lock = threading.Lock()
    
    def _read_cookie(self):
        """Read authentication cookie from file."""
        if not self.cookie_path:
            return None
        try:
            with open(self.cookie_path, 'rb') as f:
                return f.read().hex()
        except Exception as e:
            logger.warning(f"Failed to read Tor cookie: {e}")
            return None
    
    def _send_command(self, command):
        """Send a command to Tor control port and return response."""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((self.control_host, self.control_port))
            
            # Authenticate
            cookie = self._read_cookie()
            if cookie:
                auth_cmd = f"AUTHENTICATE {cookie}\r\n"
            else:
                auth_cmd = "AUTHENTICATE\r\n"
            
            sock.sendall(auth_cmd.encode())
            response = sock.recv(256).decode()
            if not response.startswith("250"):
                logger.error(f"Tor auth failed: {response.strip()}")
                sock.close()
                return None
            
            # Send command
            sock.sendall(f"{command}\r\n".encode())
            response = sock.recv(256).decode()
            
            # Quit
            sock.sendall(b"QUIT\r\n")
            sock.close()
            
            return response.strip()
        except Exception as e:
            logger.error(f"Tor control command failed: {e}")
            return None
    
    def signal_newnym(self):
        """Signal Tor to build new circuits (NEWNYM)."""
        with self._lock:
            # Respect cooldown to avoid rate limiting
            elapsed = time.time() - self.last_newnym
            if elapsed < NEWNYM_COOLDOWN:
                logger.debug(f"NEWNYM cooldown active ({NEWNYM_COOLDOWN - elapsed:.1f}s remaining)")
                return False
            
            logger.info("Signaling Tor for new circuits (NEWNYM)...")
            response = self._send_command("SIGNAL NEWNYM")
            
            if response and response.startswith("250"):
                self.last_newnym = time.time()
                logger.info("NEWNYM signal sent successfully - new circuits requested")
                return True
            else:
                logger.warning(f"NEWNYM signal failed: {response}")
                return False
    
    def get_circuit_status(self):
        """Check if Tor has established circuits."""
        response = self._send_command("GETINFO status/circuit-established")
        if response:
            return "circuit-established=1" in response
        return False


class ConnectionInfo:
    """Track metadata about a connection pair."""
    def __init__(self, client_sock, backend_sock):
        self.client_sock = client_sock
        self.backend_sock = backend_sock
        self.created_at = time.time()
        self.last_activity = time.time()
        self.bytes_sent = 0
        self.bytes_received = 0
    
    def touch(self):
        """Update last activity timestamp."""
        self.last_activity = time.time()
    
    def age(self):
        """Return connection age in seconds."""
        return time.time() - self.created_at
    
    def idle_time(self):
        """Return idle time in seconds."""
        return time.time() - self.last_activity


class CircuitManager:
    """Manages Tor circuit health and renewal with automatic NEWNYM signaling."""
    
    def __init__(self, socks_host, socks_port, target_host, target_port, tor_controller=None):
        self.socks_host = socks_host
        self.socks_port = socks_port
        self.target_host = target_host
        self.target_port = target_port
        self.tor_controller = tor_controller
        self.consecutive_failures = 0
        self.last_success = time.time()
        self.last_health_check = time.time()
        self.last_newnym = 0  # Track when we last sent NEWNYM
        self.total_connections = 0
        self.failed_connections = 0
        self._lock = threading.Lock()
    
    def record_success(self):
        """Record a successful connection."""
        with self._lock:
            self.consecutive_failures = 0
            self.last_success = time.time()
            self.total_connections += 1
    
    def record_failure(self):
        """Record a failed connection and trigger NEWNYM if needed."""
        with self._lock:
            self.consecutive_failures += 1
            self.failed_connections += 1
            if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES:
                logger.warning(f"Circuit appears unhealthy: {self.consecutive_failures} consecutive failures")
                # Trigger NEWNYM to get fresh circuits
                if self.tor_controller and time.time() - self.last_newnym > NEWNYM_COOLDOWN:
                    logger.info("Triggering NEWNYM signal to refresh Tor circuits...")
                    if self.tor_controller.signal_newnym():
                        self.last_newnym = time.time()
                        logger.info("NEWNYM signal sent - circuits will refresh")
                    else:
                        logger.warning("Failed to send NEWNYM signal")
                return True  # Signal circuit refresh needed
        return False
    
    def needs_refresh(self):
        """Check if circuit should be refreshed."""
        with self._lock:
            # Refresh if too many consecutive failures
            if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES:
                return True
            # Refresh if no successful connection in a while
            if time.time() - self.last_success > CIRCUIT_REFRESH_INTERVAL:
                return True
        return False
    
    def health_check(self):
        """Perform a health check on the circuit, triggering NEWNYM if needed."""
        logger.debug("Performing circuit health check...")
        try:
            test_sock = socks4a_connect(
                self.socks_host, self.socks_port,
                self.target_host, self.target_port,
                timeout=10
            )
            test_sock.close()
            self.record_success()
            logger.info("Circuit health check: OK")
            return True
        except Exception as e:
            logger.warning(f"Circuit health check failed: {e}")
            needs_newnym = self.record_failure()
            
            # If record_failure didn't trigger NEWNYM (e.g., within cooldown),
            # check if we've been failing for a long time and force refresh
            if needs_newnym and self.tor_controller:
                time_since_success = time.time() - self.last_success
                if time_since_success > CIRCUIT_REFRESH_INTERVAL:
                    logger.warning(f"No successful connection in {time_since_success:.0f}s, circuit likely stale")
            return False
    
    def force_circuit_refresh(self):
        """Force a circuit refresh by sending NEWNYM signal."""
        if self.tor_controller:
            logger.info("Forcing circuit refresh via NEWNYM signal...")
            if self.tor_controller.signal_newnym():
                with self._lock:
                    self.last_newnym = time.time()
                    self.consecutive_failures = 0  # Reset counter after refresh
                logger.info("Circuit refresh initiated")
                return True
            else:
                logger.error("Failed to force circuit refresh")
                return False
        else:
            logger.warning("No TorController configured, cannot refresh circuits")
            return False
    
    def get_stats(self):
        """Return circuit statistics."""
        with self._lock:
            return {
                'total_connections': self.total_connections,
                'failed_connections': self.failed_connections,
                'consecutive_failures': self.consecutive_failures,
                'last_success_ago': time.time() - self.last_success,
                'success_rate': (self.total_connections - self.failed_connections) / max(1, self.total_connections) * 100
            }


def socks4a_connect(proxy_host, proxy_port, target_host, target_port, timeout=CONNECTION_TIMEOUT):
    """
    Connect to a target via SOCKS4a proxy with configurable timeout.
    """
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(timeout)
    try:
        logger.debug(f"Connecting to SOCKS proxy at {proxy_host}:{proxy_port}")
        s.connect((proxy_host, proxy_port))
        
        # SOCKS4 Request: VN=4, CD=1 (CONNECT), DSTPORT, DSTIP (0.0.0.1 for Socks4a)
        port_bytes = struct.pack("!H", target_port)
        
        # DSTIP 0.0.0.1 for SOCKS4a signals domain resolution on proxy
        ip_bytes = socket.inet_aton("0.0.0.1")
        
        # User ID (empty) + NULL
        user_id = b"\x00"
        
        # Hostname + NULL
        hostname = target_host.encode('ascii') + b"\x00"
        
        request = b"\x04\x01" + port_bytes + ip_bytes + user_id + hostname
        s.sendall(request)
        
        # Response: VN(0), CD(90=Granted), DSTPORT, DSTIP
        response = s.recv(8)
        if len(response) < 8:
            logger.error(f"SOCKS response too short: {len(response)} bytes")
            raise Exception("Invalid SOCKS response")
            
        cd = response[1]
        
        if cd != 90:
            error_msg = f"SOCKS connection failed with code {cd}"
            if cd == 91:
                result = "Request rejected or failed"
            elif cd == 92:
                result = "SOCKS server cannot connect to identd on client"
            elif cd == 93:
                result = "Client and identd report different user-ids"
            else:
                result = "Unknown error"
            logger.error(f"{error_msg} ({result})")
            raise Exception(error_msg)
            
        logger.info(f"SOCKS tunnel established to {target_host}:{target_port}")
        s.settimeout(None)  # Remove timeout for data transfer
        return s
    except Exception as e:
        s.close()
        raise e


def close_connection_pair(sock, inputs, channels, connections):
    """Safely close a connection pair and clean up tracking."""
    if sock in inputs:
        inputs.remove(sock)
    try:
        sock.close()
    except Exception:
        pass
    
    if sock in channels:
        dest = channels[sock]
        if dest in inputs:
            inputs.remove(dest)
        try:
            dest.close()
        except Exception:
            pass
        if dest in channels:
            del channels[dest]
        del channels[sock]
    
    # Clean up connection info
    to_remove = [k for k, v in connections.items() if v.client_sock == sock or v.backend_sock == sock]
    for k in to_remove:
        del connections[k]


def cleanup_stale_connections(inputs, channels, connections):
    """Close idle and old connections."""
    now = time.time()
    stale = []
    
    for conn_id, info in connections.items():
        # Close if idle too long
        if info.idle_time() > IDLE_TIMEOUT:
            logger.debug(f"Closing idle connection (idle {info.idle_time():.0f}s)")
            stale.append(info.client_sock)
        # Close if too old (force circuit refresh for long-lived connections)
        elif info.age() > MAX_CONNECTION_AGE:
            logger.debug(f"Closing old connection (age {info.age():.0f}s)")
            stale.append(info.client_sock)
    
    for sock in stale:
        close_connection_pair(sock, inputs, channels, connections)
    
    if stale:
        logger.info(f"Cleaned up {len(stale)} stale connections")
    
    return len(stale)


def main():
    parser = argparse.ArgumentParser(description='TCP Onion Proxy with Health Monitoring and Auto-Recovery')
    parser.add_argument('--listen-port', type=int, required=True, help='Local port to listen on')
    parser.add_argument('--onion-addr', type=str, required=True, help='Target onion address')
    parser.add_argument('--onion-port', type=int, default=80, help='Target port on onion')
    parser.add_argument('--socks-host', type=str, default='127.0.0.1', help='Tor SOCKS host')
    parser.add_argument('--socks-port', type=int, default=9050, help='Tor SOCKS port')
    parser.add_argument('--tor-control-host', type=str, default='127.0.0.1', help='Tor control port host')
    parser.add_argument('--tor-control-port', type=int, default=9151, help='Tor control port')
    parser.add_argument('--tor-cookie-path', type=str, default=None,
                        help='Path to Tor control auth cookie (default: auto-detect)')
    parser.add_argument('--health-check-interval', type=int, default=HEALTH_CHECK_INTERVAL,
                        help='Health check interval in seconds')
    parser.add_argument('--verbose', '-v', action='store_true', help='Enable debug logging')
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    # Initialize Tor controller for NEWNYM signaling
    tor_controller = None
    cookie_path = args.tor_cookie_path
    if cookie_path is None:
        # Auto-detect common cookie locations
        for path in ['/tmp/fortify/tor/data/control_auth_cookie',
                     '/var/lib/tor/control_auth_cookie',
                     os.path.expanduser('~/.tor/control_auth_cookie')]:
            if os.path.exists(path):
                cookie_path = path
                break
    
    if cookie_path and os.path.exists(cookie_path):
        tor_controller = TorController(args.tor_control_host, args.tor_control_port, cookie_path)
        logger.info(f"TorController initialized (control={args.tor_control_host}:{args.tor_control_port})")
        
        # Test the control connection
        if tor_controller.get_circuit_status():
            logger.info("Tor control connection verified")
        else:
            logger.warning("Could not verify Tor control connection - NEWNYM may not work")
    else:
        logger.warning("Tor control cookie not found - automatic circuit refresh disabled")
        logger.warning("Use --tor-cookie-path to specify the cookie location")
    
    # Initialize circuit manager with Tor controller
    circuit_mgr = CircuitManager(args.socks_host, args.socks_port, args.onion_addr, args.onion_port, tor_controller)
    
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # Enable TCP keepalive on server socket
    server.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
    
    try:
        server.bind(('127.0.0.1', args.listen_port))
        server.listen(32)  # Increased backlog
        logger.info(f"Proxy listening on 127.0.0.1:{args.listen_port}")
        logger.info(f"  -> SOCKS({args.socks_host}:{args.socks_port})")
        logger.info(f"  -> {args.onion_addr}:{args.onion_port}")
        logger.info(f"Health check interval: {args.health_check_interval}s")
        logger.info(f"Auto-recovery: {'ENABLED' if tor_controller else 'DISABLED'}")
        if tor_controller:
            logger.info(f"  NEWNYM will be triggered after {MAX_CONSECUTIVE_FAILURES} consecutive failures")
        
        inputs = [server]
        channels = {}  # socket -> socket (bidirectional mapping)
        connections = {}  # id -> ConnectionInfo
        conn_counter = 0
        
        last_health_check = time.time()
        last_cleanup = time.time()
        
        # Initial health check
        logger.info("Performing initial circuit health check...")
        if circuit_mgr.health_check():
            logger.info("Initial health check passed")
        else:
            logger.warning("Initial health check failed - circuit may be unhealthy")
        
        while inputs:
            try:
                readable, _, exceptional = select.select(inputs, [], inputs, 1.0)
            except select.error as e:
                logger.error(f"Select error: {e}")
                break
            
            current_time = time.time()
            
            # Periodic health check
            if current_time - last_health_check > args.health_check_interval:
                last_health_check = current_time
                if circuit_mgr.needs_refresh():
                    logger.info("Circuit needs refresh, performing health check...")
                    circuit_mgr.health_check()
                    stats = circuit_mgr.get_stats()
                    logger.info(f"Circuit stats: {stats['total_connections']} total, "
                               f"{stats['success_rate']:.1f}% success rate")
            
            # Periodic cleanup of stale connections
            if current_time - last_cleanup > 30:  # Every 30 seconds
                last_cleanup = current_time
                cleanup_stale_connections(inputs, channels, connections)
            
            for s in list(readable):
                if s not in inputs:
                    continue  # Already removed/closed
                
                if s is server:
                    # New connection to the proxy
                    try:
                        client_conn, client_addr = server.accept()
                        logger.info(f"New client connection from {client_addr}")
                        
                        try:
                            # Connect to backend via Tor
                            backend_conn = socks4a_connect(
                                args.socks_host, args.socks_port,
                                args.onion_addr, args.onion_port
                            )
                            
                            # Enable TCP keepalive on both sockets
                            for sock in [client_conn, backend_conn]:
                                sock.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
                                # Set keepalive parameters if available (Linux)
                                try:
                                    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_KEEPIDLE, 60)
                                    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_KEEPINTVL, 10)
                                    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_KEEPCNT, 3)
                                except (AttributeError, OSError):
                                    pass  # Not available on this platform
                            
                            backend_conn.setblocking(0)
                            client_conn.setblocking(0)
                            
                            inputs.append(client_conn)
                            inputs.append(backend_conn)
                            
                            channels[client_conn] = backend_conn
                            channels[backend_conn] = client_conn
                            
                            # Track connection info
                            conn_counter += 1
                            connections[conn_counter] = ConnectionInfo(client_conn, backend_conn)
                            
                            circuit_mgr.record_success()
                            
                        except Exception as e:
                            logger.error(f"Failed to connect to backend: {e}")
                            try:
                                client_conn.close()
                            except Exception:
                                pass
                            
                            # Record failure and check if we need circuit refresh
                            if circuit_mgr.record_failure():
                                logger.info("Triggering circuit health check due to failures")
                                circuit_mgr.health_check()
                                
                    except Exception as e:
                        logger.error(f"Accept failed: {e}")
                else:
                    # Data from existing connection
                    data = None
                    try:
                        data = s.recv(16384)  # Increased buffer size
                    except (ConnectionResetError, BrokenPipeError, OSError) as e:
                        logger.debug(f"Connection error: {e}")
                        data = None
                    
                    if data:
                        # Forward data
                        if s in channels:
                            dest = channels[s]
                            try:
                                dest.sendall(data)
                                # Update connection activity
                                for info in connections.values():
                                    if info.client_sock == s or info.backend_sock == s:
                                        info.touch()
                                        if s == info.client_sock:
                                            info.bytes_sent += len(data)
                                        else:
                                            info.bytes_received += len(data)
                                        break
                            except (BrokenPipeError, ConnectionResetError, OSError) as e:
                                logger.debug(f"Forward failed: {e}")
                                data = None  # Trigger close logic below
                    
                    if not data:
                        # Close connection pair
                        logger.debug("Closing connection pair (no data)")
                        close_connection_pair(s, inputs, channels, connections)
            
            for s in list(exceptional):
                logger.warning("Socket exception detected")
                close_connection_pair(s, inputs, channels, connections)
                
    except KeyboardInterrupt:
        logger.info("Stopping proxy...")
    except Exception as e:
        logger.exception(f"Fatal error: {e}")
    finally:
        # Final stats
        stats = circuit_mgr.get_stats()
        logger.info(f"Final stats: {stats['total_connections']} connections, "
                   f"{stats['success_rate']:.1f}% success rate")
        
        try:
            server.close()
        except Exception:
            pass
        # Only clean up inputs if it was initialized
        if 'inputs' in dir():
            for s in list(inputs):
                try:
                    s.close()
                except Exception:
                    pass
        logger.info("Proxy stopped")


if __name__ == '__main__':
    main()
