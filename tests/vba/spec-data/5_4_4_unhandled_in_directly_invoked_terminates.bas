' INV: vbal.5.4.4, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 directly invoked procedure has Terminate policy initially
' EXPECT: before
' ERROR: 11
Public Sub Main()
    Dim x As Long
    Debug.Print "before"
    x = 1 / 0
    Debug.Print "after"
End Sub
