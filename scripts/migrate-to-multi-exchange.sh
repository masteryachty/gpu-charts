#!/bin/bash

# Multi-Exchange Logger Data Migration Script
# This script migrates existing data from the old structure to the new multi-exchange structure

set -e

# Configuration
DATA_PATH=${DATA_PATH:-"/mnt/md/data"}
BACKUP_PATH=${BACKUP_PATH:-"/mnt/md/data.backup"}
DRY_RUN=${DRY_RUN:-false}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --data-path PATH    Path to data directory (default: /mnt/md/data)"
    echo "  --backup-path PATH  Path for backup (default: /mnt/md/data.backup)"
    echo "  --dry-run          Show what would be done without making changes"
    echo "  --skip-backup      Skip backup step (not recommended)"
    echo "  --force            Force migration even if coinbase directory exists"
    echo "  --help             Show this help message"
}

# Parse arguments
SKIP_BACKUP=false
FORCE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --data-path)
            DATA_PATH="$2"
            shift 2
            ;;
        --backup-path)
            BACKUP_PATH="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --skip-backup)
            SKIP_BACKUP=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Validation
if [ ! -d "$DATA_PATH" ]; then
    log_error "Data directory does not exist: $DATA_PATH"
    exit 1
fi

# Check if coinbase directory already exists
if [ -d "$DATA_PATH/coinbase" ] && [ "$FORCE" != "true" ]; then
    log_error "Directory $DATA_PATH/coinbase already exists!"
    log_error "This might indicate migration has already been done."
    log_error "Use --force to override this check."
    exit 1
fi

# Check for any symbol directories
SYMBOL_DIRS=$(find "$DATA_PATH" -maxdepth 1 -type d -name "*-*" | head -10)
if [ -z "$SYMBOL_DIRS" ]; then
    log_warning "No symbol directories found in $DATA_PATH"
    log_warning "Expected directories like BTC-USD, ETH-USD, etc."
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

log_info "Migration Configuration:"
log_info "  Data Path: $DATA_PATH"
log_info "  Backup Path: $BACKUP_PATH"
log_info "  Dry Run: $DRY_RUN"
log_info "  Skip Backup: $SKIP_BACKUP"
log_info ""

# Step 1: Backup
if [ "$SKIP_BACKUP" != "true" ] && [ "$DRY_RUN" != "true" ]; then
    log_info "Creating backup..."
    
    if [ -d "$BACKUP_PATH" ]; then
        log_warning "Backup directory already exists: $BACKUP_PATH"
        read -p "Remove existing backup? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$BACKUP_PATH"
        else
            log_error "Cannot proceed with existing backup directory"
            exit 1
        fi
    fi
    
    log_info "Copying data to $BACKUP_PATH (this may take a while)..."
    cp -a "$DATA_PATH" "$BACKUP_PATH"
    log_success "Backup created successfully"
else
    if [ "$DRY_RUN" = "true" ]; then
        log_info "[DRY RUN] Would create backup at $BACKUP_PATH"
    else
        log_warning "Skipping backup - this is not recommended!"
    fi
fi

# Step 2: Create coinbase directory
COINBASE_DIR="$DATA_PATH/coinbase"
if [ "$DRY_RUN" = "true" ]; then
    log_info "[DRY RUN] Would create directory: $COINBASE_DIR"
else
    log_info "Creating coinbase directory..."
    mkdir -p "$COINBASE_DIR"
    log_success "Created $COINBASE_DIR"
fi

# Step 3: Move symbol directories
log_info "Moving symbol directories to coinbase folder..."
MOVED_COUNT=0

for dir in "$DATA_PATH"/*; do
    if [ -d "$dir" ]; then
        basename=$(basename "$dir")
        
        # Skip if it's already an exchange directory
        if [ "$basename" = "coinbase" ] || [ "$basename" = "binance" ]; then
            continue
        fi
        
        # Check if it looks like a symbol directory (contains hyphen)
        if [[ "$basename" == *"-"* ]]; then
            if [ "$DRY_RUN" = "true" ]; then
                log_info "[DRY RUN] Would move: $dir -> $COINBASE_DIR/$basename"
            else
                log_info "Moving: $basename"
                mv "$dir" "$COINBASE_DIR/"
            fi
            ((MOVED_COUNT++))
        else
            log_warning "Skipping non-symbol directory: $basename"
        fi
    fi
done

log_success "Moved $MOVED_COUNT symbol directories"

# Step 4: Verify migration
if [ "$DRY_RUN" != "true" ]; then
    log_info "Verifying migration..."
    
    # Check if coinbase directory has content
    COINBASE_SYMBOLS=$(find "$COINBASE_DIR" -maxdepth 1 -type d -name "*-*" | wc -l)
    if [ "$COINBASE_SYMBOLS" -eq 0 ]; then
        log_error "No symbol directories found in $COINBASE_DIR after migration"
        log_error "Migration may have failed!"
        exit 1
    fi
    
    log_success "Found $COINBASE_SYMBOLS symbols in coinbase directory"
    
    # Show sample of migrated structure
    log_info "Sample of new structure:"
    tree -L 3 "$COINBASE_DIR" | head -20 || ls -la "$COINBASE_DIR" | head -10
fi

# Step 5: Create marker file
if [ "$DRY_RUN" != "true" ]; then
    MARKER_FILE="$DATA_PATH/.migration_completed"
    echo "Migration completed at $(date)" > "$MARKER_FILE"
    echo "Version: multi-exchange-v1" >> "$MARKER_FILE"
    echo "Backup location: $BACKUP_PATH" >> "$MARKER_FILE"
    log_success "Created migration marker file"
fi

# Summary
echo ""
if [ "$DRY_RUN" = "true" ]; then
    log_info "DRY RUN COMPLETE - No changes were made"
    log_info "Run without --dry-run to perform actual migration"
else
    log_success "Migration completed successfully!"
    log_info ""
    log_info "Next steps:"
    log_info "1. Update logger configuration to use new multi-exchange logger"
    log_info "2. Start the new logger service"
    log_info "3. Verify data is being written to correct locations"
    log_info "4. Once verified, you can remove the backup: rm -rf $BACKUP_PATH"
    log_info ""
    log_warning "The old coinbase-logger should be stopped before starting the new logger"
fi

# Rollback instructions
echo ""
log_info "If you need to rollback:"
log_info "1. Stop the new logger"
log_info "2. Remove coinbase directory: rm -rf $DATA_PATH/coinbase"
log_info "3. Restore from backup: mv $BACKUP_PATH/* $DATA_PATH/"
log_info "4. Start the old coinbase-logger"