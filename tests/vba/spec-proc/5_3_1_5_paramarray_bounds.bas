' INV: vbal.5.3.1.5, decl.paramarray
' SOURCE: spec ms-vbal 5.3.1.11 param array has lower bound 0; empty one has upper bound -1
' EXPECT: lb=0 ub=-1
' EXPECT: lb=0 ub=2 first=a last=c
Option Base 1
Sub P(ParamArray v() As Variant)
    If UBound(v) < LBound(v) Then
        Debug.Print "lb=" & CStr(LBound(v)) & " ub=" & CStr(UBound(v))
    Else
        Debug.Print "lb=" & CStr(LBound(v)) & " ub=" & CStr(UBound(v)) & " first=" & v(0) & " last=" & v(2)
    End If
End Sub
Public Sub Main()
    P
    P "a", "b", "c"
End Sub
