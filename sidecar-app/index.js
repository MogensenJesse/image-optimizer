const command = process.argv[2];

switch (command) {
  case 'ping':
    const message = process.argv[3];
    console.log(`pong, ${message}`);
    break;
  default:
    console.error(`unknown command ${command}`);
    process.exit(1);
}