' INV: vbal.6.1.2.3.1.3, func.ccur
' SOURCE: spec ms-vbal 6.1.2.3.1.3 CCur coercion
' EXPECT: v=2.5
' EXPECT: int=7
' EXPECT: true=-1
' EXPECT: str=1.25
' EXPECT: type=Currency
' EXPECT: over=6
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "v=" & CStr(CCur(2.5))
    Debug.Print "int=" & CStr(CCur(7))
    Debug.Print "true=" & CStr(CCur(True))
    Debug.Print "str=" & CStr(CCur("1.25"))
    Debug.Print "type=" & TypeName(CCur(1))
    On Error Resume Next
    Err.Clear: v = CCur(1E+15): Debug.Print "over=" & Err.Number
    Err.Clear: v = CCur(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CCur("abc"): Debug.Print "text=" & Err.Number
End Sub
