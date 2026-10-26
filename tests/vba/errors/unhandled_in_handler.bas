' INV: stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/on-error-statement
' ERROR: 11
Public Sub Main()
    On Error GoTo H
    Error 5
H:
    Error 11
End Sub
