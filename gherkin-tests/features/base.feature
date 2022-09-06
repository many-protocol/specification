Feature: ping server

Scenario: Server is running
  Given the server has a heartbeat
  And the server has a status
  And the server has endpoints
