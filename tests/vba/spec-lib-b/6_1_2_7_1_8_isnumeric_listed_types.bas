' INV: vbal.6.1.2.7.1.8, func.isnumeric
' SOURCE: spec ms-vbal 6.1.2.7.1.8 IsNumeric true for the listed numeric value types
' EXPECT: byte=True
' EXPECT: integer=True
' EXPECT: long=True
' EXPECT: single=True
' EXPECT: double=True
' EXPECT: currency=True
' EXPECT: decimal=True
' EXPECT: boolean=True
Public Sub Main()
    Debug.Print "byte=" & CStr(IsNumeric(CByte(1)))
    Debug.Print "integer=" & CStr(IsNumeric(CInt(1)))
    Debug.Print "long=" & CStr(IsNumeric(CLng(1)))
    Debug.Print "single=" & CStr(IsNumeric(CSng(1.5)))
    Debug.Print "double=" & CStr(IsNumeric(CDbl(1.5)))
    Debug.Print "currency=" & CStr(IsNumeric(CCur(1.5)))
    Debug.Print "decimal=" & CStr(IsNumeric(CDec(1.5)))
    Debug.Print "boolean=" & CStr(IsNumeric(True))
End Sub
