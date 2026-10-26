' INV: vbal.6.1.2.3.1.6, func.cdec
' SOURCE: spec ms-vbal 6.1.2.3.1.6 CDec coercion to Decimal
' EXPECT: type=Decimal
' EXPECT: int=7
' EXPECT: str=1.5
' EXPECT: true=-1
' EXPECT: err=13
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "type=" & TypeName(CDec(1))
    Debug.Print "int=" & CStr(CDec(7))
    Debug.Print "str=" & CStr(CDec("1.5"))
    Debug.Print "true=" & CStr(CDec(True))
    On Error Resume Next
    Err.Clear: v = CDec(CVErr(5)): Debug.Print "err=" & Err.Number
    Err.Clear: v = CDec(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CDec("abc"): Debug.Print "text=" & Err.Number
End Sub
