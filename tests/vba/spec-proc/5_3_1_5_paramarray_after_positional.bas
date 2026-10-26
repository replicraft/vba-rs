' INV: vbal.5.3.1.5, decl.paramarray
' SOURCE: spec ms-vbal 5.3.1.5 param array holds only the extra arguments after positional ones
' EXPECT: f=1 n=2 sum=5
Sub P(first As Long, ParamArray rest() As Variant)
    Dim i As Long, s As Long
    For i = LBound(rest) To UBound(rest)
        s = s + rest(i)
    Next
    Debug.Print "f=" & CStr(first) & " n=" & CStr(UBound(rest) + 1) & " sum=" & CStr(s)
End Sub
Public Sub Main()
    P 1, 2, 3
End Sub
