' INV: vbal.6.1.2.11.1.35, func.split
' SOURCE: spec ms-vbal 6.1.2.11.1.35 Split of a zero-length string gives an empty array
' EXPECT: lb=0
' EXPECT: ub=-1
' EXPECT: t=String()
Public Sub Main()
    Dim a() As String
    a = Split("", ",")
    Debug.Print "lb=" & CStr(LBound(a))
    Debug.Print "ub=" & CStr(UBound(a))
    Debug.Print "t=" & TypeName(Split("", ","))
End Sub
