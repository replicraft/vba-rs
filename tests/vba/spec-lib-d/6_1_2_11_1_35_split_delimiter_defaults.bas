' INV: vbal.6.1.2.11.1.35, func.split
' SOURCE: spec ms-vbal 6.1.2.11.1.35 Split default delimiter is a space; zero-length delimiter gives one element
' EXPECT: sp=3 a|b|c
' EXPECT: zl=1 [a,b]
' EXPECT: adj=3 [a][][b]
Public Sub Main()
    Dim a() As String
    a = Split("a b c")
    Debug.Print "sp=" & CStr(UBound(a) + 1) & " " & Join(a, "|")
    a = Split("a,b", "")
    Debug.Print "zl=" & CStr(UBound(a) - LBound(a) + 1) & " [" & a(0) & "]"
    a = Split("a,,b", ",")
    Debug.Print "adj=" & CStr(UBound(a) + 1) & " [" & a(0) & "][" & a(1) & "][" & a(2) & "]"
End Sub
