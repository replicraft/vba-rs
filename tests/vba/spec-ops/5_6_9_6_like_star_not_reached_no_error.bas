' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like star check happens only when the star element is encountered
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("x" Like "b*[")
End Sub
