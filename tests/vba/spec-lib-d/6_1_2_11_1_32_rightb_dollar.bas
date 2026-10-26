' INV: vbal.6.1.2.11.1.32
' SOURCE: derived ms-vbal 6.1.2.11.1.32 RightB$ counts bytes from the end; a character takes two bytes
' EXPECT: a=[c] 2
' EXPECT: b=[bc] 4
Public Sub Main()
    Dim s As String
    s = RightB$("abc", 2)
    Debug.Print "a=[" & s & "] " & CStr(LenB(s))
    s = RightB$("abc", 4)
    Debug.Print "b=[" & s & "] " & CStr(LenB(s))
End Sub
