' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like invalid pattern error only raised if matching reaches it
' EXPECT: False
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("x" Like "a[")
    Debug.Print CStr("" Like "a[")
End Sub
