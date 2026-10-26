' INV: vbal.6.1.1.16
' SOURCE: spec ms-vbal 6.1.1.16 VbVarType values
' EXPECT: vbArray=8192
' EXPECT: vbBoolean=11
' EXPECT: vbByte=17
' EXPECT: vbCurrency=6
' EXPECT: vbDataObject=13
' EXPECT: vbDate=7
' EXPECT: vbDecimal=14
' EXPECT: vbDouble=5
' EXPECT: vbEmpty=0
' EXPECT: vbError=10
' EXPECT: vbInteger=2
' EXPECT: vbLong=3
' EXPECT: vbLongLong=20
' EXPECT: vbNull=1
' EXPECT: vbObject=9
' EXPECT: vbSingle=4
' EXPECT: vbString=8
' EXPECT: vbUserDefinedType=36
' EXPECT: vbVariant=12
Public Sub Main()
    Debug.Print "vbArray=" & CStr(vbArray)
    Debug.Print "vbBoolean=" & CStr(vbBoolean)
    Debug.Print "vbByte=" & CStr(vbByte)
    Debug.Print "vbCurrency=" & CStr(vbCurrency)
    Debug.Print "vbDataObject=" & CStr(vbDataObject)
    Debug.Print "vbDate=" & CStr(vbDate)
    Debug.Print "vbDecimal=" & CStr(vbDecimal)
    Debug.Print "vbDouble=" & CStr(vbDouble)
    Debug.Print "vbEmpty=" & CStr(vbEmpty)
    Debug.Print "vbError=" & CStr(vbError)
    Debug.Print "vbInteger=" & CStr(vbInteger)
    Debug.Print "vbLong=" & CStr(vbLong)
    Debug.Print "vbLongLong=" & CStr(vbLongLong)
    Debug.Print "vbNull=" & CStr(vbNull)
    Debug.Print "vbObject=" & CStr(vbObject)
    Debug.Print "vbSingle=" & CStr(vbSingle)
    Debug.Print "vbString=" & CStr(vbString)
    Debug.Print "vbUserDefinedType=" & CStr(vbUserDefinedType)
    Debug.Print "vbVariant=" & CStr(vbVariant)
End Sub
