' INV: vbal.6.1.2.11.1.7, func.filter
' SOURCE: spec ms-vbal 6.1.2.11.1.7 Filter returns a zero-based array
' EXPECT: lb=0
' EXPECT: n=2
Option Base 1
Public Sub Main()
    Dim src As Variant, r As Variant
    src = Array("a1", "b", "a2")
    r = Filter(src, "a")
    Debug.Print "lb=" & CStr(LBound(r))
    Debug.Print "n=" & CStr(UBound(r) - LBound(r) + 1)
End Sub
