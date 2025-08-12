// User service module

/**
 * Get user by ID
 * @param {string} id - User ID 
 * @param {boolean} includeDetails - Whether to include extra details
 * @returns {Object} User object
 */
export function getUserById(id, includeDetails = false) {
    const user = { id: id, name: 'Test User' };
    if (includeDetails) {
        user.details = { created: new Date(), active: true };
    }
    return user;
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