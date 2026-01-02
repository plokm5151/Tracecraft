FROM rust:latest

WORKDIR /app

# Copy the entire project
COPY . .

# Ensure scripts are executable
RUN chmod +x gen_advanced_tests.sh scripts/smoke_test.sh

# Install any necessary dependencies if needed (e.g., for graphviz if verify script used dot command, but here we just check file existence)
# RUN apt-get update && apt-get install -y ...

# Set the default command to run the smoke test
CMD ["./scripts/smoke_test.sh"]
