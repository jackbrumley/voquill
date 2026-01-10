#!/usr/bin/env node

/**
 * Development script for Voquill
 * Runs the app with hot reload - restarts on file changes
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

// Colors for console output
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    magenta: '\x1b[35m',
    cyan: '\x1b[36m'
};

function log(message, color = colors.reset) {
    console.log(`${color}${message}${colors.reset}`);
}

function logStep(step, message) {
    log(`\n${colors.bright}[${step}]${colors.reset} ${colors.cyan}${message}${colors.reset}`);
}

function logSuccess(message) {
    log(`${colors.green}✅ ${message}${colors.reset}`);
}

function logError(message) {
    log(`${colors.red}❌ ${message}${colors.reset}`);
}

function runCommand(command, cwd = process.cwd()) {
    try {
        log(`   ${colors.blue}$ ${command}${colors.reset}`);
        execSync(command, { 
            cwd, 
            stdio: 'inherit',
            encoding: 'utf8'
        });
    } catch (error) {
        logError(`Command failed: ${command}`);
        process.exit(1);
    }
}

function checkBasicRequirements() {
    logStep('1', 'Checking basic requirements...');
    
    if (!fs.existsSync('src') || !fs.existsSync('src/ui/package.json')) {
        logError('This script must be run from the project root directory');
        logError('Expected structure: ./src/ui/package.json');
        process.exit(1);
    }
    
    try {
        const nodeVersion = execSync('node --version', { encoding: 'utf8' }).trim();
        logSuccess(`Node.js: ${nodeVersion}`);
    } catch (error) {
        logError('Node.js not found. Please install from https://nodejs.org/');
        process.exit(1);
    }
    
    try {
        const rustVersion = execSync('rustc --version', { encoding: 'utf8' }).trim();
        logSuccess(`Rust: ${rustVersion}`);
    } catch (error) {
        logError('Rust not found. Please install from https://rustup.rs/');
        process.exit(1);
    }
    
    try {
        const tauriVersion = execSync('cargo tauri --version', { encoding: 'utf8' }).trim();
        logSuccess(`Tauri CLI: ${tauriVersion}`);
    } catch (error) {
        logError('Tauri CLI not found. Install with: cargo install tauri-cli');
        process.exit(1);
    }
}

function installDependencies() {
    logStep('2', 'Installing dependencies (if needed)...');
    
    const uiDir = path.join('src', 'ui');
    const nodeModules = path.join(uiDir, 'node_modules');
    
    if (!fs.existsSync(nodeModules)) {
        runCommand('npm install', uiDir);
        logSuccess('Dependencies installed');
    } else {
        logSuccess('Dependencies already installed');
    }
}

function runDev() {
    logStep('3', 'Starting development server...');
    
    log(`\n${colors.cyan}The app will open in a new window.${colors.reset}`);
    log(`${colors.cyan}Changes to files will trigger automatic rebuild and restart.${colors.reset}`);
    log(`${colors.yellow}Press Ctrl+C to stop the development server.${colors.reset}\n`);
    
    const srcDir = path.join('src');
    runCommand('cargo tauri dev', srcDir);
}

function main() {
    log(`${colors.bright}${colors.magenta}🚀 Voquill Development Server${colors.reset}`);
    log(`${colors.cyan}Run app with hot reload${colors.reset}\n`);
    
    try {
        checkBasicRequirements();
        installDependencies();
        runDev();
    } catch (error) {
        logError(`Dev server failed: ${error.message}`);
        process.exit(1);
    }
}

if (require.main === module) {
    main();
}
