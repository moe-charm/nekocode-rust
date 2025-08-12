// Profile service module
import { getUserById } from './user.js';

/**
 * Get user profile
 * @param {string} userId - User ID
 * @returns {Object} User profile
 */
export function getUserProfile(userId) {
    const user = getUserById(userId);
    return {
        ...user,
        profile: {
            avatar: 'avatar.png',
            bio: 'User bio'
        }
    };
}

/**
 * Update user profile
 * @param {string} userId - User ID
 * @param {Object} profileData - Profile data
 * @returns {Object} Updated profile
 */
export function updateUserProfile(userId, profileData) {
    const user = getUserById(userId);
    return {
        ...user,
        profile: {
            ...user.profile,
            ...profileData
        }
    };
}