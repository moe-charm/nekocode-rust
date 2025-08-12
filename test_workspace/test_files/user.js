// User service module

/**
 * Get user by ID
 * @param {string} id - User ID 
 * @returns {Object} User object
 */
export function getUserById(id) {
    return { id: id, name: 'Test User' };
}

/**
 * Update user data
 * @param {string} id - User ID
 * @param {Object} data - User data to update
 * @returns {Object} Updated user
 */
export function updateUser(id, data) {
    return { id: id, ...data };
}

/**
 * Create new user
 * @param {Object} userData - User data
 * @returns {Object} Created user
 */
export function createUser(userData) {
    return { id: 'new-id', ...userData };
}