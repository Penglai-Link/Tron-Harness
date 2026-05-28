## Frontend Tech Stack

The frontend follows a **domain-driven frontend** architecture.
Application bootstrapping, routing, domain modules, shared UI, global state, and internationalization are separated by responsibility so that route features can evolve without turning shared components or app shell code into domain-specific dumping grounds.

### Stack

- **React**: latest stable version
- **React Router**: route matching, browser history integration, redirects, and routable page composition
- **Zustand**: lightweight global frontend state for app-shell UI and cross-route client state
- **Vite 8**: fast frontend build tool and development server
- **TypeScript**: latest stable version
- **PreText**: text/state-oriented UI abstraction  
  - https://github.com/chenglou/pretext
- **shadcn/ui**: component system built on Radix UI and Tailwind CSS
- **i18next + react-i18next**: React internationalization runtime and translation hook/component layer
- **npm**: package management
- **reactbit**: dynamic login background
- **
