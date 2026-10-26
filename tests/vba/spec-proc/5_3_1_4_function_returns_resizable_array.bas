' INV: vbal.5.3.1.4
' SOURCE: spec ms-vbal 5.3.1.4 function-type with array designator yields a resizable array
' EXPECT: lb=0 ub=2
' EXPECT: v=30
Function MakeArr() As Long()
    Dim r() As Long
    ReDim r(0 To 2)
    r(2) = 30
    MakeArr = r
End Function
Public Sub Main()
    Dim a() As Long
    a = MakeArr()
    Debug.Print "lb=" & CStr(LBound(a)) & " ub=" & CStr(UBound(a))
    Debug.Print "v=" & CStr(a(2))
End Sub
