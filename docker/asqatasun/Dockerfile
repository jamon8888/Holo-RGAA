FROM asqatasun/asqatasun-server-standalone:6.0.0-rc.6

RUN apk add --no-cache firefox-esr xvfb

# Create symlink for Firefox at expected location
RUN mkdir -p /opt/firefox && ln -sf /usr/bin/firefox-esr /opt/firefox/firefox

# Create a custom entrypoint that starts Xvfb first
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
