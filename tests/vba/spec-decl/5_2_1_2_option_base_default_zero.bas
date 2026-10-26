' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 default lower bound is 0
' EXPECT: lb=0
' EXPECT: ub=3
Dim a(3) As Long
Public Sub Main()
    Debug.Print "lb=" & CStr(LBound(a))
    Debug.Print "ub=" & CStr(UBound(a))
End Sub
