' INV: stmt.resume, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.2 Resume statement with a label
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
