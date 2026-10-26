' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, largest Currency value is a valid literal; integer digits with @
' EXPECT: 922337203685477.5807
' EXPECT: 100
Public Sub Main()
    Debug.Print CStr(922337203685477.5807@)
    Debug.Print CStr(1E2@)
End Sub
