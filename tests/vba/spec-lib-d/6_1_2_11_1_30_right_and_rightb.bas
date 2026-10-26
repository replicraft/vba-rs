' INV: vbal.6.1.2.11.1.30
' SOURCE: derived ms-vbal 6.1.2.11.1.30 Right takes characters from the end, RightB bytes
' EXPECT: a=[de]
' EXPECT: b=[c] 2
' EXPECT: c=[bc] 4
' EXPECT: d=[abc]
Public Sub Main()
    Debug.Print "a=[" & Right("abcde", 2) & "]"
    Dim s As String
    s = RightB("abc", 2)
    Debug.Print "b=[" & s & "] " & CStr(LenB(s))
    s = RightB("abc", 4)
    Debug.Print "c=[" & s & "] " & CStr(LenB(s))
    Debug.Print "d=[" & RightB("abc", 100) & "]"
End Sub
