' INV: vbal.6.1.2.3.1.5, func.cdbl
' SOURCE: spec ms-vbal 6.1.2.3.1.5 CDbl coercion, numeric-coercion-string grammar
' EXPECT: int=7
' EXPECT: true=-1
' EXPECT: date=1.5
' EXPECT: str=1.5
' EXPECT: exp=150
' EXPECT: dexp=100
' EXPECT: ws=12
' EXPECT: plus=3
' EXPECT: empty=0
' EXPECT: type=Double
' EXPECT: over=6
' EXPECT: null=94
' EXPECT: text=13
' EXPECT: emptyStr=13
Public Sub Main()
    Dim v As Variant
    Dim e As Variant
    Debug.Print "int=" & CStr(CDbl(7))
    Debug.Print "true=" & CStr(CDbl(True))
    Debug.Print "date=" & CStr(CDbl(CDate(1.5)))
    Debug.Print "str=" & CStr(CDbl("1.5"))
    Debug.Print "exp=" & CStr(CDbl("1.5e2"))
    Debug.Print "dexp=" & CStr(CDbl("1d2"))
    Debug.Print "ws=" & CStr(CDbl("  12  "))
    Debug.Print "plus=" & CStr(CDbl("+3"))
    Debug.Print "empty=" & CStr(CDbl(e))
    Debug.Print "type=" & TypeName(CDbl(1))
    On Error Resume Next
    Err.Clear: v = CDbl("1e400"): Debug.Print "over=" & Err.Number
    Err.Clear: v = CDbl(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CDbl("abc"): Debug.Print "text=" & Err.Number
    Err.Clear: v = CDbl(""): Debug.Print "emptyStr=" & Err.Number
End Sub
