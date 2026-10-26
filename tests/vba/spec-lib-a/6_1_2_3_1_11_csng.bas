' INV: vbal.6.1.2.3.1.11, func.csng
' SOURCE: spec ms-vbal 6.1.2.3.1.11 CSng coercion
' EXPECT: int=7
' EXPECT: half=0.5
' EXPECT: str=1.5
' EXPECT: exp=1500
' EXPECT: true=-1
' EXPECT: empty=0
' EXPECT: type=Single
' EXPECT: over=6
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Dim e As Variant
    Debug.Print "int=" & CStr(CSng(7))
    Debug.Print "half=" & CStr(CSng(0.5))
    Debug.Print "str=" & CStr(CSng("1.5"))
    Debug.Print "exp=" & CStr(CSng("1.5e3"))
    Debug.Print "true=" & CStr(CSng(True))
    Debug.Print "empty=" & CStr(CSng(e))
    Debug.Print "type=" & TypeName(CSng(1))
    On Error Resume Next
    Err.Clear: v = CSng(1E+39): Debug.Print "over=" & Err.Number
    Err.Clear: v = CSng(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CSng("zz"): Debug.Print "text=" & Err.Number
End Sub
