#!/bin/sh
set -ex

# Install Firefox, Xvfb, and download geckodriver
apk add --no-cache firefox-esr xvfb xorg-server-common curl

# Download geckodriver
curl -sL https://github.com/mozilla/geckodriver/releases/download/v0.35.0/geckodriver-v0.35.0-linux64.tar.gz | tar xz -C /opt/
chmod +x /opt/geckodriver

# Create Firefox wrapper with --no-sandbox at expected location
mkdir -p /opt/firefox
cat > /opt/firefox/firefox << 'WRAPPER'
#!/bin/sh
set -ex
echo "Firefox wrapper called with args: $@" > /tmp/firefox-wrapper.log
exec /usr/bin/firefox-esr --no-sandbox "$@"
WRAPPER
chmod +x /opt/firefox/firefox

# Start Xvfb
export DISPLAY=:99
Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp &
sleep 2

# Start the application
exec java ${JAVA_OPTS} -Dapp.logging.path="${ASQA_LOG_DIR}" -jar /home/asqatasun/asqatasun-server.jar "$@"