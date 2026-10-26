' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 redim of unmatched name declares a resizable array
' EXPECT: lb=0 ub=3
Public Sub Main()
    ReDim a(3)
    Debug.Print "lb=" & CStr(LBound(a)) & " ub=" & CStr(UBound(a))
End Sub
