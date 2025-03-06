const fs = require('fs');
const path = require('path');

// Configuration
const JSON_OUTPUT_FILE = path.join(__dirname, 'sidecar-output.json');
const REPORT_FILE = path.join(__dirname, 'sidecar-analysis.md');

// Load captured data
function loadData() {
  try {
    if (!fs.existsSync(JSON_OUTPUT_FILE)) {
      console.error(`Error: Output file not found: ${JSON_OUTPUT_FILE}`);
      console.error('Please run the capture test first');
      process.exit(1);
    }
    
    const data = JSON.parse(fs.readFileSync(JSON_OUTPUT_FILE, 'utf8'));
    console.log(`Loaded ${data.length} messages from ${JSON_OUTPUT_FILE}`);
    return data;
  } catch (error) {
    console.error('Error loading data:', error);
    process.exit(1);
  }
}

// Generate analysis report
function generateReport(data) {
  // Start building the report
  let report = `# Sidecar Communication Analysis Report\n\n`;
  report += `*Generated on: ${new Date().toISOString()}*\n\n`;
  
  // Overall statistics
  report += `## Summary Statistics\n\n`;
  report += `- **Total messages captured**: ${data.length}\n`;
  
  // Count messages by type
  const messageTypeCount = {};
  data.forEach(item => {
    if (item.parsed && item.parsed.type) {
      const type = item.parsed.type;
      messageTypeCount[type] = (messageTypeCount[type] || 0) + 1;
    }
  });
  
  report += `- **Message types**:\n`;
  Object.entries(messageTypeCount).forEach(([type, count]) => {
    report += `  - \`${type}\`: ${count} messages (${((count / data.length) * 100).toFixed(1)}%)\n`;
  });
  
  // Analyze progress messages
  const progressMessages = data.filter(item => 
    item.parsed && item.parsed.type === 'progress');
  
  if (progressMessages.length > 0) {
    report += `\n## Progress Messages Analysis\n\n`;
    
    // Count by progress type
    const progressTypeCount = {};
    progressMessages.forEach(item => {
      if (item.parsed && item.parsed.progressType) {
        const type = item.parsed.progressType;
        progressTypeCount[type] = (progressTypeCount[type] || 0) + 1;
      }
    });
    
    report += `- **Progress message types**:\n`;
    Object.entries(progressTypeCount).forEach(([type, count]) => {
      report += `  - \`${type}\`: ${count} messages (${((count / progressMessages.length) * 100).toFixed(1)}%)\n`;
    });
    
    // Sample of each progress type
    report += `\n### Progress Message Samples\n\n`;
    Object.keys(progressTypeCount).forEach(type => {
      const sample = progressMessages.find(item => 
        item.parsed && item.parsed.progressType === type);
      
      if (sample) {
        report += `#### \`${type}\` Sample:\n\n`;
        report += '```json\n';
        report += JSON.stringify(sample.parsed, null, 2);
        report += '\n```\n\n';
      }
    });
  }
  
  // Analyze results messages
  const resultsMessages = data.filter(item => 
    item.parsed && item.parsed.type === 'results');
  
  if (resultsMessages.length > 0) {
    report += `\n## Results Messages Analysis\n\n`;
    report += `- **Total results messages**: ${resultsMessages.length}\n\n`;
    
    // Sample of a results message
    if (resultsMessages.length > 0) {
      const sample = resultsMessages[0];
      report += `### Results Message Sample:\n\n`;
      report += '```json\n';
      report += JSON.stringify(sample.parsed, null, 2);
      report += '\n```\n\n';
    }
  }
  
  // Analyze other message types
  const otherTypes = Object.keys(messageTypeCount)
    .filter(type => type !== 'progress' && type !== 'results');
  
  if (otherTypes.length > 0) {
    report += `\n## Other Message Types\n\n`;
    
    otherTypes.forEach(type => {
      const typeMessages = data.filter(item => 
        item.parsed && item.parsed.type === type);
      
      if (typeMessages.length > 0) {
        report += `### \`${type}\` Messages (${typeMessages.length})\n\n`;
        
        // Sample of this message type
        const sample = typeMessages[0];
        report += `Sample:\n\n`;
        report += '```json\n';
        report += JSON.stringify(sample.parsed, null, 2);
        report += '\n```\n\n';
      }
    });
  }
  
  // Communication flow diagram (text-based)
  report += `\n## Communication Flow\n\n`;
  report += '```\n';
  report += 'Sharp Sidecar                      Rust Backend\n';
  report += '-------------                      ------------\n';
  report += '      |                                 |\n';
  
  // Group by the first 5 messages, then summarize
  const firstItems = Math.min(5, data.length);
  for (let i = 0; i < firstItems; i++) {
    const item = data[i];
    const type = item.parsed?.type || 'unknown';
    const arrow = ' -------> ';
    report += `      | ${type}${arrow}               |\n`;
    report += `      |                                 |\n`;
  }
  
  if (data.length > 5) {
    report += `      | ...more messages...             |\n`;
    report += `      |                                 |\n`;
    
    // Add last message
    const lastItem = data[data.length - 1];
    const lastType = lastItem.parsed?.type || 'unknown';
    const lastArrow = ' -------> ';
    report += `      | ${lastType}${lastArrow}               |\n`;
  }
  
  report += '```\n\n';
  
  // Recommendations based on analysis
  report += `\n## Data Size Analysis\n\n`;
  
  // Calculate total size of data transferred
  let totalSize = 0;
  data.forEach(item => {
    totalSize += item.message.length;
  });
  
  report += `- **Total data transferred**: ${formatSize(totalSize)}\n`;
  report += `- **Average message size**: ${formatSize(totalSize / data.length)}\n`;
  
  // Find the largest messages
  const sortedBySize = [...data].sort((a, b) => b.message.length - a.message.length);
  const largestMessages = sortedBySize.slice(0, 3);
  
  report += `\n### Largest Messages\n\n`;
  largestMessages.forEach((item, index) => {
    const type = item.parsed?.type || 'unknown';
    const subtype = item.parsed?.progressType || '';
    report += `${index + 1}. **Type**: \`${type}${subtype ? '/' + subtype : ''}\` - **Size**: ${formatSize(item.message.length)}\n`;
  });
  
  return report;
}

// Helper function to format size in bytes to human-readable format
function formatSize(bytes) {
  if (bytes < 1024) {
    return `${bytes} bytes`;
  } else if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(2)} KB`;
  } else {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
}

// Main function
function main() {
  console.log('Analyzing sidecar communication data...');
  
  const data = loadData();
  const report = generateReport(data);
  
  // Write report to file
  fs.writeFileSync(REPORT_FILE, report);
  console.log(`Analysis report saved to: ${REPORT_FILE}`);
}

// Run the main function
main(); 