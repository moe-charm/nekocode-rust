// API routes module
import { getUserById, updateUser } from './user.js';
import { getUserProfile } from './profile.js';

/**
 * Handle GET /api/users/:id
 * @param {string} userId - User ID
 * @returns {Object} API response
 */
export function handleGetUser(userId) {
    const user = getUserById(userId);
    const profile = getUserProfile(userId);
    
    return {
        status: 200,
        data: {
            user,
            profile
        }
    };
}

/**
 * Handle PUT /api/users/:id  
 * @param {string} userId - User ID
 * @param {Object} data - Update data
 * @returns {Object} API response
 */
export function handleUpdateUser(userId, data) {
    const updatedUser = updateUser(userId, data);
    
    return {
        status: 200,
        data: updatedUser
    };
}