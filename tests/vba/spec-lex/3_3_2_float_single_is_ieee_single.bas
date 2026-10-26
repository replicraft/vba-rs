' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, ! suffix gives a Single value (32-bit), # gives Double
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(CDbl(0.1!) = 0.1)
    Debug.Print CStr(CDbl(0.1#) = 0.1)
End Sub
