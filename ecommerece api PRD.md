
2. Users
Action	Route	Role	Request	Response
GET	/users	admin	—	List of all users
GET	/users/:id	admin	—	Single user details
PATCH	/users/:id	admin	{fields}	Updated user
DELETE	/users/:id	admin	—	User deleted
GET	/profile	user	—	Current user profile
PATCH	/profile	user	{fields}	Updated user profile
3. Products
Action	Route	Role	Request	Response
GET	/products	admin/user	—	List of products
GET	/products/:id	admin/user	—	Product details
POST	/products	admin	{name, price, category, stock, images, description}	Created product
PATCH	/products/:id	admin	{fields}	Updated product
DELETE	/products/:id	admin	—	Product deleted
GET	/products/category/:category	admin/user	—	Products by category
GET	/products/search	admin/user	query params	Filtered product list
4. Categories
Action	Route	Role	Request	Response
GET	/categories	admin/user	—	List of categories
GET	/categories/:id	admin/user	—	Single category details
POST	/categories	admin	{name, description}	Created category
PATCH	/categories/:id	admin	{fields}	Updated category
DELETE	/categories/:id	admin	—	Category deleted
5. Cart
Action	Route	Role	Request	Response
GET	/cart	user	—	Current user cart
POST	/cart	user	{productId, quantity}	Item added to cart
PATCH	/cart/:itemId	user	{quantity}	Updated cart item
DELETE	/cart/:itemId	user	—	Removed item from cart
DELETE	/cart	user	—	Clear cart
6. Orders
Action	Route	Role	Request	Response
POST	/orders	user	{cartId, shippingAddress, paymentMethod}	Created order
GET	/orders	admin	—	All orders
GET	/orders	user	—	Current user orders
GET	/orders/:id	admin/user	—	Single order details
PATCH	/orders/:id/status	admin	{status}	Updated order status
DELETE	/orders/:id	admin	—	Order deleted (rare)
7. Reviews & Ratings
Action	Route	Role	Request	Response
POST	/products/:id/reviews	user	{rating, comment}	Added review
GET	/products/:id/reviews	admin/user	—	List of reviews
PATCH	/reviews/:id	admin/user	{comment, rating}	Updated review
DELETE	/reviews/:id	admin/user	—	Deleted review
8. Payments (optional)
Action	Route	Role	Request	Response
POST	/payments/checkout	user	{orderId, paymentDetails}	Payment status
GET	/payments/:id	admin/user	—	Payment details
9. Discounts & Coupons
Action	Route	Role	Request	Response
GET	/coupons	admin/user	—	List of coupons
POST	/coupons	admin	{code, discount, expiry}	Created coupon
PATCH	/coupons/:id	admin	{fields}	Updated coupon
DELETE	/coupons/:id	admin	—	Coupon deleted
