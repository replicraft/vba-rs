' INV: stmt.resume, stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/resume-statement
' EXPECT: done
Public Sub Main()
    On Error GoTo H
    Error 5
    Debug.Print "skipped"
Done:
    Debug.Print "done"
    Exit Sub
H:
    Resume Done
End Sub
