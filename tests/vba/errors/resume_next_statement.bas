' INV: stmt.resume, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.2 Resume Next statement
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
