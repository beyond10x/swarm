import { createRouter, createWebHistory } from 'vue-router'

// Route table owned by the integrator. View files are owned as noted; owners overwrite their file
// and never this table.
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'splash', component: () => import('./views/SplashView.vue') },                       // W1
    { path: '/swarm/:id', name: 'swarm', component: () => import('./views/SwarmView.vue'), props: true }, // W1 (embeds SwarmCanvas, W2)
    { path: '/swarm/:id/flow/:flowId', name: 'flow', component: () => import('./views/FlowView.vue'), props: true }, // W2
    { path: '/ui', name: 'ui', component: () => import('./views/ComponentsView.vue') },                   // W3
  ],
})
