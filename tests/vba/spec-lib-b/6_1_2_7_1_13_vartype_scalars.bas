' INV: vbal.6.1.2.7.1.13, func.vartype
' SOURCE: spec ms-vbal 6.1.2.7.1.13 VarType values for scalar value types
' EXPECT: boolean=11
' EXPECT: byte=17
' EXPECT: currency=6
' EXPECT: date=7
' EXPECT: decimal=14
' EXPECT: double=5
' EXPECT: empty=0
' EXPECT: error=10
' EXPECT: integer=2
' EXPECT: long=3
' EXPECT: longlong=20
' EXPECT: null=1
' EXPECT: single=4
' EXPECT: string=8
Public Sub Main()
    Debug.Print "boolean=" & CStr(VarType(True))
    Debug.Print "byte=" & CStr(VarType(CByte(1)))
    Debug.Print "currency=" & CStr(VarType(CCur(1)))
    Debug.Print "date=" & CStr(VarType(DateSerial(2020, 1, 2)))
    Debug.Print "decimal=" & CStr(VarType(CDec(1)))
    Debug.Print "double=" & CStr(VarType(CDbl(1)))
    Debug.Print "empty=" & CStr(VarType(Empty))
    Debug.Print "error=" & CStr(VarType(CVErr(5)))
    Debug.Print "integer=" & CStr(VarType(CInt(1)))
    Debug.Print "long=" & CStr(VarType(CLng(1)))
    Debug.Print "longlong=" & CStr(VarType(CLngLng(1)))
    Debug.Print "null=" & CStr(VarType(Null))
    Debug.Print "single=" & CStr(VarType(CSng(1)))
    Debug.Print "string=" & CStr(VarType("s"))
End Sub
