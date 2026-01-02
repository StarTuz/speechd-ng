# SysV Init Guide (Slackware, Gentoo, etc.)

Since `speechd-ng` is designed as a modern daemon, it defaults to running in the foreground (standard for Systemd/Docker). To run it on SysV systems, you use `start-stop-daemon` to handle backgrounding.

## 1. Installation

Copy the binary to a global location:

```bash
cp target/release/speechd-ng /usr/local/bin/speechd-ng
chmod +x /usr/local/bin/speechd-ng
```

## 2. Init Script Template (/etc/init.d/speechd-ng)

Create the file `/etc/init.d/speechd-ng` (or `/etc/rc.d/rc.speechd-ng` on Slackware) with the following content:

```bash
#!/bin/sh
#
# speechd-ng      Start/Stop the Next-Gen Speech Daemon
#

DAEMON=/usr/local/bin/speechd-ng
NAME=speechd-ng
DESC="SpeechD-NG Service"
PIDFILE=/var/run/$NAME.pid
USER=root  # Or a specific user like 'speech'

case "$1" in
  start)
    echo "Starting $DESC: $NAME"
    /sbin/start-stop-daemon --start --background --make-pidfile --pidfile $PIDFILE \
        --chuid $USER --exec $DAEMON -- 
    ;;
  stop)
    echo "Stopping $DESC: $NAME"
    /sbin/start-stop-daemon --stop --pidfile $PIDFILE --retry 5
    rm -f $PIDFILE
    ;;
  restart)
    $0 stop
    sleep 1
    $0 start
    ;;
  status)
    if [ -f $PIDFILE ]; then
        echo "$NAME is running (PID $(cat $PIDFILE))"
    else
        echo "$NAME is not running"
    fi
    ;;
  *)
    echo "Usage: $0 {start|stop|restart|status}"
    exit 1
    ;;
esac

exit 0
```

## 3. Permissions

Make the script executable:

```bash
chmod +x /etc/init.d/speechd-ng
```

## 4. Slackware Specifics

On Slackware, place the script in `/etc/rc.d/rc.speechd-ng`.
Add the following to `/etc/rc.d/rc.local` to start on boot:

```bash
if [ -x /etc/rc.d/rc.speechd-ng ]; then
  /etc/rc.d/rc.speechd-ng start
fi
```

## 5. Security Note regarding SysV

Unlike Systemd, SysV does not provide automatic sandboxing (PrivateTmp, etc.).
To secure the daemon on SysV:

1. Run as a dedicated user (create `speech` user).
2. Use `jail` or `chroot` approaches if paranoia is required.
3. Ensure `/tmp` has restricted permissions only if needed.
