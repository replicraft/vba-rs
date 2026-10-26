' INV: vbal.6.1.2.3.1.14, func.cverr
' SOURCE: spec ms-vbal 6.1.2.3.1.14 CVErr argument range 0 to 65535
' EXPECT: neg=5
' EXPECT: over=5
' EXPECT: big=5
' EXPECT: ok0=0
' EXPECT: ok65535=0
' EXPECT: text=13
' EXPECT: null=94
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CVErr(-1): Debug.Print "neg=" & Err.Number
    Err.Clear: v = CVErr(65536): Debug.Print "over=" & Err.Number
    Err.Clear: v = CVErr(100000): Debug.Print "big=" & Err.Number
    Err.Clear: v = CVErr(0): Debug.Print "ok0=" & Err.Number
    Err.Clear: v = CVErr(65535): Debug.Print "ok65535=" & Err.Number
    Err.Clear: v = CVErr("abc"): Debug.Print "text=" & Err.Number
    Err.Clear: v = CVErr(Null): Debug.Print "null=" & Err.Number
End Sub
