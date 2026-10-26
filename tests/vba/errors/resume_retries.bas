' INV: stmt.resume, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.2 Resume statement re-executes the faulting statement, 5.6.9.3.6 \ operator
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
