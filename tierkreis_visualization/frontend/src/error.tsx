// import { Link, useRouteError, isRouteErrorResponse } from "react-router";
// import MyLogo from "./quantinuum_logo.svg"; //

// export default function ErrorPage() {
//   const error = useRouteError();
//   let errorMessage: string;
//   if (isRouteErrorResponse(error)) {
//     errorMessage = error.statusText;
//   } else if (error instanceof Error) {
//     errorMessage = error.message;
//   } else if (typeof error === "string") {
//     errorMessage = error;
//   } else {
//     errorMessage = "An unknown error occurred";
//   }

//   return (
//     <div style={containerStyle}>
//       <div style={contentStyle}>
//         <h1>Oops! 😱</h1>
//         <p>Sorry, an unexpected error has occurred.</p>
//         <p>
//           <i>{errorMessage}</i>
//         </p>
//         <Link to="/">Go back to the homepage</Link>
//       </div>
//       <footer style={footerStyle}>
//         <img src={MyLogo} alt="Company Logo" style={{ width: "500px" }} />
//       </footer>
//     </div>
//   );
// }

// // Basic inline styles for layout
// // In a real app, you'd likely use CSS classes
// const containerStyle: React.CSSProperties = {
//   display: "flex",
//   flexDirection: "column",
//   alignItems: "center",
//   justifyContent: "center",
//   minHeight: "100vh",
//   textAlign: "center",
//   padding: "20px",
//   boxSizing: "border-box",
// };

// const contentStyle: React.CSSProperties = {
//   flexGrow: 1,
//   display: "flex",
//   flexDirection: "column",
//   justifyContent: "center",
// };

// const footerStyle: React.CSSProperties = {
//   marginTop: "auto",
//   padding: "20px 0",
// };
