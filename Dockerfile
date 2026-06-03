FROM scratch
COPY teaql-forge-server-bin /app/teaql-forge-server
EXPOSE 8080
ENTRYPOINT ["/app/teaql-forge-server", "--host", "0.0.0.0", "--port", "8080"]
