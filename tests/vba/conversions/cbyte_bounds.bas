' INV: func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte
' EXPECT: 0
' EXPECT: 255
Public Sub Main()
    Debug.Print CStr(CByte(0))
    Debug.Print CStr(CByte(255))
End Sub
