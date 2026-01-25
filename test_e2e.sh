#!/bin/bash

# Intent Compiler - End-to-End API Test Script for Ecommerce API
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

echo -e "${BLUE}🚀 Starting E2E tests for Ecommerce API...${NC}"

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
echo -e "\n${YELLOW}--- ADMIN SETUP ---${NC}"

# A1. Admin Register
echo -e "${BLUE}[A1] Registering Admin User...${NC}"
RESPONSE=$(curl -L -s -X POST "$BASE_URL/users/auth/register" \
  -H "Content-Type: application/json" \
  -w "\n%{http_code}" \
  -d "{
    \"email\": \"$ADMIN_EMAIL\",
    \"password\": \"$ADMIN_PASS\",
    \"name\": \"Super Admin\",
    \"role\": \"admin\"
  }")
CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')
assert_status "200" "$CODE" "Admin registered" "$BODY"

echo -e "${BLUE}[A2] Admin Login...${NC}"
ADMIN_TOKEN=$(curl -L -s -X POST "$BASE_URL/users/auth/login" \
  -H "Content-Type: application/json" \
  -d "{ \"email\": \"$ADMIN_EMAIL\", \"password\": \"$ADMIN_PASS\" }" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

if [ -n "$ADMIN_TOKEN" ]; then
  echo -e "${GREEN}  ✅ Admin Token captured${NC}"
else
  echo -e "${RED}  ❌ Admin Login failed${NC}"
  curl -L -s -X POST "$BASE_URL/users/auth/login" \
    -H "Content-Type: application/json" \
    -d "{ \"email\": \"$ADMIN_EMAIL\", \"password\": \"$ADMIN_PASS\" }"
  exit 1
fi

# ==============================================================================
# CATEGORIES & PRODUCTS
# ==============================================================================
echo -e "\n${YELLOW}--- CATEGORIES & PRODUCTS ---${NC}"

# 1. Create Category
echo -e "${BLUE}[1] Create Category (Admin)...${NC}"
CAT_RESPONSE=$(curl -L -s -X POST "$BASE_URL/categorys/categories" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"name\": \"Electronics $UNIQUE_ID\", \"description\": \"Gadgets and gizmos\" }")
CAT_ID=$(echo $CAT_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$CAT_ID" ]; then
  echo -e "${GREEN}  ✅ Category created (ID: $CAT_ID)${NC}"
else
  echo -e "${RED}  ❌ Category creation failed: $CAT_RESPONSE${NC}"
fi

# 2. Create Product
echo -e "${BLUE}[2] Create Product (Admin)...${NC}"
PROD_RESPONSE=$(curl -L -s -X POST "$BASE_URL/products/" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Pro Laptop $UNIQUE_ID\",
    \"description\": \"High performance laptop\",
    \"price\": 1299.99,
    \"stock\": 50,
    \"category_id\": \"$CAT_ID\"
  }")
PRODUCT_ID=$(echo $PROD_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$PRODUCT_ID" ]; then
  echo -e "${GREEN}  ✅ Product created (ID: $PRODUCT_ID)${NC}"
else
  echo -e "${RED}  ❌ Product creation failed: $PROD_RESPONSE${NC}"
fi

# ==============================================================================
# USER HAPPY PATHS
# ==============================================================================
echo -e "\n${YELLOW}--- USER ACTIONS ---${NC}"

# 3. User Register & Login
echo -e "${BLUE}[3] User Register & Login...${NC}"
curl -L -s -X POST "$BASE_URL/users/auth/register" \
  -H "Content-Type: application/json" \
  -d "{ \"email\": \"$USER_EMAIL\", \"password\": \"$USER_PASS\", \"name\": \"Tester\" }" > /dev/null

USER_LOGIN_RES=$(curl -L -s -X POST "$BASE_URL/users/auth/login" \
  -H "Content-Type: application/json" \
  -d "{ \"email\": \"$USER_EMAIL\", \"password\": \"$USER_PASS\" }")
USER_TOKEN=$(echo $USER_LOGIN_RES | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

if [ -n "$USER_TOKEN" ]; then
  echo -e "${GREEN}  ✅ User Token captured${NC}"
else
  echo -e "${RED}  ❌ User Login failed: $USER_LOGIN_RES${NC}"
fi

# 4. Profile
echo -e "${BLUE}[4] Get Profile...${NC}"
PROFILE_RESPONSE=$(curl -L -s -X GET "$BASE_URL/users/profile" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -w "\n%{http_code}")
CODE=$(echo "$PROFILE_RESPONSE" | tail -n1)
BODY=$(echo "$PROFILE_RESPONSE" | sed '$d')
assert_status "200" "$CODE" "Retrieved profile" "$BODY"

# 5. Cart
echo -e "${BLUE}[5] Add to Cart...${NC}"
CART_RESPONSE=$(curl -L -s -X POST "$BASE_URL/cartitems/cart" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"product_id\": \"$PRODUCT_ID\", \"quantity\": 2 }")
CART_ITEM_ID=$(echo $CART_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$CART_ITEM_ID" ]; then
  echo -e "${GREEN}  ✅ Added to cart (ID: $CART_ITEM_ID)${NC}"
else
  echo -e "${RED}  ❌ Add to cart failed: $CART_RESPONSE${NC}"
fi

# 6. Orders
echo -e "${BLUE}[6] Create Order...${NC}"
ORDER_RESPONSE=$(curl -L -s -X POST "$BASE_URL/orders/" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"total\": 2599.98,
    \"shipping_address\": \"123 Test St\",
    \"payment_method\": \"credit_card\"
  }")
ORDER_ID=$(echo $ORDER_RESPONSE | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$ORDER_ID" ]; then
  echo -e "${GREEN}  ✅ Order created (ID: $ORDER_ID)${NC}"
else
  echo -e "${RED}  ❌ Order creation failed: $ORDER_RESPONSE${NC}"
fi

# 7. Reviews
echo -e "${BLUE}[7] Create Review...${NC}"
REVIEW_PATH="/reviews/products/$PRODUCT_ID/reviews"
RESPONSE=$(curl -L -s -X POST "$BASE_URL$REVIEW_PATH" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -w "\n%{http_code}" \
  -d "{ \"product_id\": \"$PRODUCT_ID\", \"rating\": 5, \"comment\": \"Amazing!\" }")
CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')
assert_status "200" "$CODE" "Created review" "$BODY"

# ==============================================================================
# SAD PATHS
# ==============================================================================
echo -e "\n${YELLOW}--- SAD PATHS ---${NC}"

# S1. Unauthenticated
echo -e "${BLUE}[S1] Unauthenticated Cart Access...${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X GET "$BASE_URL/cartitems/cart")
assert_status "401" "$CODE" "Rejected unauthenticated access"

# S2. Unauthorized
echo -e "${BLUE}[S2] Unauthorized Category Creation...${NC}"
CODE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/categorys/categories" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{ \"name\": \"Hack\", \"description\": \"Hack\" }")
assert_status "403" "$CODE" "Rejected unauthorized access"


echo -e "\n\n${GREEN}✅ E2E tests completed.${NC}"
