#!/bin/bash

# Intent Compiler - End-to-End API Test Script
# Tests the examples/app.intent application with Happy and Sad paths

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:18000"
UNIQUE_ID=$(date +%s)

# Test Data
ADMIN_EMAIL="admin-$UNIQUE_ID@example.com"
ADMIN_PASS="admin123"
USER_EMAIL="tester-$UNIQUE_ID@example.com"
USER_PASS="password123"

echo -e "${BLUE}🚀 Starting E2E tests for Intent Compiler...${NC}"

# Helper function for assertions
assert_status() {
  local expected=$1
  local actual=$2
  local msg=$3
  local response=$4
  if [ "$expected" == "$actual" ]; then
    echo -e "${GREEN}  ✅ [PASS] $msg (Status: $actual)${NC}"
  else
    echo -e "${RED}  ❌ [FAIL] $msg (Expected: $expected, Actual: $actual)${NC}"
    if [ -n "$response" ]; then
      echo -e "${RED}     Response: $response${NC}"
    fi
  fi
}

# 0. Health Check
echo -n "Checking server health... "
HEALTH_STATUS=$(curl -L -s -o /dev/null -w "%{http_code}" "$BASE_URL/health")
if [ "$HEALTH_STATUS" != "200" ]; then
  echo -e "${RED}FAILED${NC}"
  echo -e "Error: Server is not running at $BASE_URL or not healthy."
  exit 1
fi
echo -e "${GREEN}OK${NC}"

# ==============================================================================
# SETUP & ADMIN ACTIONS
# ==============================================================================
echo -e "\n${YELLOW}--- ADMIN ACTIONS ---${NC}"

# A1. Admin Signup
echo -e "${BLUE}[A1] Creating Admin User...${NC}"
RESPONSE=$(curl -L -s -X POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -w "\n%{http_code}" \
  -d "{
    \"email\": \"$ADMIN_EMAIL\",
    \"password\": \"$ADMIN_PASS\",
    \"full_name\": \"Super Admin\",
    \"role\": \"admin\"
  }")
CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')
assert_status "200" "$CODE" "Admin signed up" "$BODY"

# A2. Admin Login
echo -e "${BLUE}[A2] Admin Login...${NC}"
ADMIN_TOKEN=$(curl -L -s -X POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{ \"email\": \"$ADMIN_EMAIL\", \"password\": \"$ADMIN_PASS\" }" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

if [ -n "$ADMIN_TOKEN" ]; then
  echo -e "${GREEN}  ✅ Admin Token captured${NC}"
else
  echo -e "${RED}  ❌ Admin Login failed${NC}"
  exit 1
fi

# A3. Create Product (Admin)
echo -e "${BLUE}[A3] Create Product (Admin)...${NC}"
PRODUCT_RESPONSE=$(curl -L -s -X POST "$BASE_URL/products/" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "E2E Master Product",
    "description": "Initial description",
    "price": 99.99,
    "stock": 100,
    "category": "Electronics"
  }')
PRODUCT_ID=$(echo $PRODUCT_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$PRODUCT_ID" ]; then
    echo -e "${GREEN}  ✅ Product created (ID: $PRODUCT_ID)${NC}"
else
    echo -e "${RED}  ❌ Product creation failed${NC}"
    exit 1
fi

# A4. Update Product (Admin)
echo -e "${BLUE}[A4] Update Product (Admin)...${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X PATCH "$BASE_URL/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"id\": \"$PRODUCT_ID\", \"price\": 79.99 }")
assert_status "200" "$CODE" "Admin updated product"

# ==============================================================================
# HAPPY PATHS (NORMAL USER)
# ==============================================================================
echo -e "\n${YELLOW}--- HAPPY PATHS (USER) ---${NC}"

# 1. Signup
echo -e "\n${BLUE}[1] Happy Path: User Signup${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASS\",
    \"full_name\": \"Regular Tester\"
  }")
assert_status "200" "$CODE" "User signed up successfully"

# 2. Login
echo -e "\n${BLUE}[2] Happy Path: User Login${NC}"
LOGIN_RESPONSE=$(curl -L -s -X POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASS\"
  }")
TOKEN=$(echo $LOGIN_RESPONSE | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')
if [ -n "$TOKEN" ]; then
  echo -e "${GREEN}  ✅ Captured JWT Token${NC}"
else
  echo -e "${RED}  ❌ Failed to capture Token${NC}"
fi

# 3. Get Me
echo -e "\n${BLUE}[3] Happy Path: Get Me${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/users/me" \
  -H "Authorization: Bearer $TOKEN")
assert_status "200" "$CODE" "User retrieved own profile"

# 4. Create Order
echo -e "\n${BLUE}[4] Happy Path: Create Order${NC}"
ORDER_RESPONSE=$(curl -L -s -X POST "$BASE_URL/orders/" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"total\": 510.50 }")
ORDER_ID=$(echo $ORDER_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$ORDER_ID" ]; then
  echo -e "${GREEN}  ✅ Order created (ID: $ORDER_ID)${NC}"
else
  echo -e "${RED}  ❌ Failed to create order${NC}"
fi

# 5. Get Specific Order
echo -e "\n${BLUE}[5] Happy Path: Get Specific Order${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/orders/$ORDER_ID" \
  -H "Authorization: Bearer $TOKEN")
assert_status "200" "$CODE" "Retrieved specific order"

# 6. Cancel Order
echo -e "\n${BLUE}[6] Happy Path: Cancel Order${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/orders/$ORDER_ID/cancel" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"id\": \"$ORDER_ID\" }")
assert_status "200" "$CODE" "Order cancelled successfully"

# 7. Create Review
echo -e "\n${BLUE}[7] Happy Path: Create Review${NC}"
REVIEW_RESPONSE=$(curl -L -s -X POST "$BASE_URL/reviews/" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"product_id\": \"$PRODUCT_ID\", \"rating\": 5, \"comment\": \"Best product ever!\" }")
REVIEW_ID=$(echo $REVIEW_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$REVIEW_ID" ]; then
    echo -e "${GREEN}  ✅ Review created (ID: $REVIEW_ID)${NC}"
else
    echo -e "${RED}  ❌ Failed to create review${NC}"
fi

# 8. List Reviews
echo -e "\n${BLUE}[8] Happy Path: List Reviews${NC}"
REVIEWS_LIST=$(curl -L -s -X GET "$BASE_URL/reviews/")
if [[ "$REVIEWS_LIST" == *"$REVIEW_ID"* ]]; then
    echo -e "${GREEN}  ✅ Review found in global list${NC}"
else
    echo -e "${RED}  ❌ Review not found in list${NC}"
fi

# 9. Get Specific Review
echo -e "\n${BLUE}[9] Happy Path: Get Specific Review${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/reviews/$REVIEW_ID")
assert_status "200" "$CODE" "Retrieved specific review"

# 10. Delete Review (User)
echo -e "\n${BLUE}[10] Happy Path: Delete Review${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE_URL/reviews/$REVIEW_ID" \
  -H "Authorization: Bearer $TOKEN")
assert_status "200" "$CODE" "Review deleted"

# ==============================================================================
# CLEANUP & ADMIN DELETE
# ==============================================================================
echo -e "\n${YELLOW}--- CLEANUP ACTIONS ---${NC}"

# C1. Delete Product (Admin)
echo -e "${BLUE}[C1] Delete Product (Admin)...${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE_URL/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN")
assert_status "200" "$CODE" "Admin deleted product"

# ==============================================================================
# SAD PATHS
# ==============================================================================
echo -e "\n${YELLOW}--- SAD PATHS ---${NC}"

# S1. Unauthenticated Access
echo -e "\n${BLUE}[S1] Sad Path: Unauthenticated Access to Profile${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/users/me")
assert_status "401" "$CODE" "Rejected unauthenticated access"

# S2. Policy Check: Admin Only
echo -e "\n${BLUE}[S2] Sad Path: Admin Only Route by Normal User${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/products/" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Hacker Item",
    "description": "Fail",
    "price": 0.0,
    "stock": 0
  }')
assert_status "403" "$CODE" "Policy AdminOnly enforced"

# S3. Policy Check: OwnsOrder
echo -e "\n${BLUE}[S3] Sad Path: Access Other's Order${NC}"
# Use Admin Token to try to access User's Order (Admin is not the owner)
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/orders/$ORDER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN")
assert_status "403" "$CODE" "Policy OwnsOrder enforced"

echo -e "\n\n${GREEN}✅ Extended E2E tests completed.${NC}"
