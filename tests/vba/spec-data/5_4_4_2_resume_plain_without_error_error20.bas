' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume with no active error is error 20
' EXPECT: x
' ERROR: 20
Public Sub Main()
    Debug.Print "x"
    Resume
End Sub
