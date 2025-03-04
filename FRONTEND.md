# Frontend Documentation

## Overview

This React-based frontend is built for a Tauri desktop application that optimizes images. The application uses a modular component structure and follows a well-organized styling system using SCSS with BEM methodology.

## Styling System Structure

The styling system follows a modular and hierarchical organization:

```
src/assets/styles/
├── base/                  # Base styles and variables
│   ├── reset.scss         # CSS reset
│   ├── typography.scss    # Typography styles
│   └── variables.scss     # Global variables
├── app/                   # App-wide styles
│   └── _app.scss          # Main application styles
├── components/            # Component-specific styles
│   ├── _TitleBar.scss     # TitleBar component styles
│   ├── _ProgressBar.scss  # ProgressBar component styles
│   └── FloatingMenu.scss  # FloatingMenu component styles
└── main.scss              # Main style entry file
```

### Import System

The main.scss file serves as the entry point for all styles, importing other SCSS files in a structured manner:

```scss
// Base styles
@forward 'base/variables';
@use 'base/reset';
@use 'base/typography';

// App styles
@use 'app/app';

// Component styles
@use 'components/FloatingMenu';
@use 'components/ProgressBar';
@use 'components/TitleBar';
```

## Reset File

The project uses a comprehensive `reset.scss` file that establishes baseline styles across the application. This file:

- Sets `box-sizing: border-box` for all elements
- Resets margins and paddings to zero
- Standardizes form elements and typography
- Addresses accessibility concerns like reduced motion preferences
- Normalizes HTML5 element behaviors

**Note**: Some component files override or duplicate these reset properties. This is sometimes intentional for component-specific styling, but redundancies have been cleaned up where appropriate.

## BEM Methodology Implementation

The project employs the Block, Element, Modifier (BEM) methodology for CSS class naming and structure. This creates a clear, maintainable relationship between HTML and CSS.

### Naming Convention

- **Block**: Standalone entity that is meaningful on its own (e.g., `.title-bar`, `.menu`)
- **Element**: Parts of a block with no standalone meaning (e.g., `.title-bar-title`, `.menu-item`)
- **Modifier**: Flags on blocks or elements for changing appearance or behavior (e.g., `.menu-overlay--active`)

### SCSS Nesting with BEM

The project leverages SCSS nesting with the `&` parent selector to implement BEM in a clean, readable way:

```scss
.title-bar {
  display: flex;
  justify-content: space-between;
  
  &-title {  // Compiles to .title-bar-title
    color: #ffffff;
    font-size: 14px;
  }
}

.window-control {
  &-button {  // Compiles to .window-control-button
    width: 30px;
    height: 30px;
    
    &:hover {  // Compiles to .window-control-button:hover
      background-color: rgba(255, 255, 255, 0.1);
    }
  }
  
  &-close {  // Compiles to .window-control-close
    &:hover {  // Compiles to .window-control-close:hover
      background-color: #e81123;
    }
  }
}
```

### Modifiers

Modifiers use the `--` syntax and are often applied conditionally in the React components:

```scss
.menu-overlay {
  background-color: rgba(0, 0, 0, 0);
  
  &--active {  // Compiles to .menu-overlay--active
    background-color: rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(8px);
  }
}
```

## Component Structure

React components are organized in a modular fashion in the `/src/components` directory. Each component typically has:

1. A corresponding SCSS file in `/src/assets/styles/components/`
2. BEM class naming that matches the component's structure
3. Clean separation of concerns between UI structure and styling

## Best Practices Used

1. **Prefixing of Partials**: Underscore prefix (`_component.scss`) for files meant to be imported rather than compiled directly
2. **SCSS Features**: Leverages nesting, variables, and parent selectors
3. **Modular Design**: Styles are separated by component and purpose
4. **Consistent Naming**: Follows BEM convention throughout the codebase
5. **Responsive Techniques**: Uses flexible units and responsive design principles

## Common UI Patterns

- **Custom window controls**: Implemented in TitleBar for the Tauri application
- **Floating menus**: Contextual menus with backdrop blur and animations
- **Progress indicators**: Visual feedback for processing operations
- **Drag-and-drop interface**: Core interaction pattern for the image optimization app
