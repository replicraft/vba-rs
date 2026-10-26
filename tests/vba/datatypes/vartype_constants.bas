' INV: func.vartype
' SOURCE: spec ms-vbal 6.1.1.16 VbVarType
' EXPECT: 0
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
' EXPECT: 4
' EXPECT: 5
' EXPECT: 6
' EXPECT: 7
' EXPECT: 8
' EXPECT: 9
' EXPECT: 10
' EXPECT: 11
' EXPECT: 12
' EXPECT: 13
' EXPECT: 14
' EXPECT: 17
' EXPECT: 8192
Public Sub Main()
    Debug.Print CStr(vbEmpty)
    Debug.Print CStr(vbNull)
    Debug.Print CStr(vbInteger)
    Debug.Print CStr(vbLong)
    Debug.Print CStr(vbSingle)
    Debug.Print CStr(vbDouble)
    Debug.Print CStr(vbCurrency)
    Debug.Print CStr(vbDate)
    Debug.Print CStr(vbString)
    Debug.Print CStr(vbObject)
    Debug.Print CStr(vbError)
    Debug.Print CStr(vbBoolean)
    Debug.Print CStr(vbVariant)
    Debug.Print CStr(vbDataObject)
    Debug.Print CStr(vbDecimal)
    Debug.Print CStr(vbByte)
    Debug.Print CStr(vbArray)
End Sub
