' INV: vbal.6.1.2.11.1.37, func.strconv
' SOURCE: spec ms-vbal 6.1.2.11.1.37 StrConv proper case word separators: tab, linefeed, vertical tab, form feed, carriage return, space, NUL
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: proper case also capitalises after a hyphen
' EXPECT: tab=A/B
' EXPECT: lf=A/B
' EXPECT: vt=A/B
' EXPECT: ff=A/B
' EXPECT: cr=A/B
' EXPECT: nul=A/B
' EXPECT: non=Ab-cd
Public Sub Main()
    Dim s As String
    s = StrConv("a" & vbTab & "b", vbProperCase, 1033)
    Debug.Print "tab=" & Replace(s, vbTab, "/")
    s = StrConv("a" & vbLf & "b", vbProperCase, 1033)
    Debug.Print "lf=" & Replace(s, vbLf, "/")
    s = StrConv("a" & Chr$(11) & "b", vbProperCase, 1033)
    Debug.Print "vt=" & Replace(s, Chr$(11), "/")
    s = StrConv("a" & Chr$(12) & "b", vbProperCase, 1033)
    Debug.Print "ff=" & Replace(s, Chr$(12), "/")
    s = StrConv("a" & vbCr & "b", vbProperCase, 1033)
    Debug.Print "cr=" & Replace(s, vbCr, "/")
    s = StrConv("a" & Chr$(0) & "b", vbProperCase, 1033)
    Debug.Print "nul=" & Replace(s, Chr$(0), "/")
    Debug.Print "non=" & StrConv("ab-cd", vbProperCase, 1033)
End Sub
