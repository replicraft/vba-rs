' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 fixed array elements reset to initial value
' EXPECT: 0|
' EXPECT: lb=1 ub=3
Public Sub Main()
    Dim a(1 To 3) As Long
    Dim s(1 To 3) As String
    a(2) = 9
    s(2) = "x"
    Erase a, s
    Debug.Print CStr(a(2)) & "|" & s(2)
    Debug.Print "lb=" & CStr(LBound(a)) & " ub=" & CStr(UBound(a))
End Sub
