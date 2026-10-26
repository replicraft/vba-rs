' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 line number label 0 requires no label definition
' EXPECT: ok
Public Sub Main()
    On Error GoTo 0
    Debug.Print "ok"
End Sub
