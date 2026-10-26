' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 named arguments map by parameter name, in any order, after positional ones
' EXPECT: a=1 b=2 c=3
' EXPECT: a=4 b=9 c=6
Sub S(a As Long, Optional b As Long = 9, Optional c As Long = 9)
    Debug.Print "a=" & CStr(a) & " b=" & CStr(b) & " c=" & CStr(c)
End Sub
Public Sub Main()
    S c:=3, b:=2, a:=1
    S 4, c:=6
End Sub
