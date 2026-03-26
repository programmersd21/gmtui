# Security Policy

## Token Storage

`gmtui` stores OAuth tokens in `~/.config/gmtui/token.json` by default. On Unix systems the file is created with `0600` permissions to restrict access to your user account. Treat this file like a password.

If you rotate Google OAuth credentials or suspect a leak, delete the token file and re-authenticate. You can also revoke access from the Google account security console.

## OAuth Credentials

Your Client ID and Client Secret are only stored locally in `~/.config/gmtui/config.toml`. Do not commit them. If you think they are exposed, rotate the credentials in Google Cloud Console and update your local config.

## Reporting a Vulnerability

Email: `geniussantu1983@gmail.com`

Please include:
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or screenshots

## Disclosure Timeline

- Acknowledgement within 72 hours
- Initial assessment within 7 days
- Fix release as soon as practical

Name: Soumalya Das  
Email: geniussantu1983@gmail.com  
GitHub: https://github.com/programmersd21  
Repo: https://github.com/programmersd21/gmtui
