' INV: stmt.resume, stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/resume-statement
' EXPECT: h
' EXPECT: continued
Public Sub Main()
    On Error GoTo H
    Error 5
    Debug.Print "continued"
    Exit Sub
H:
    Debug.Print "h"
    Resume Next
End Sub
