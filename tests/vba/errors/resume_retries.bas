' INV: stmt.resume, stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/resume-statement
' EXPECT:  2 
' EXPECT: done
Public Sub Main()
    Dim n As Long
    On Error GoTo H
    n = 0
    Debug.Print 10 \ n
    Debug.Print "done"
    Exit Sub
H:
    n = 5
    Resume
End Sub
