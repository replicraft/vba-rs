' INV: vbal.6.1.2.11.1.21
' SOURCE: derived ms-vbal 6.1.2.11.1.21 LeftB$ counts bytes; a character takes two bytes of the UTF-16 representation
' EXPECT: a=[a] 2
' EXPECT: b=[ab] 4
' EXPECT: c=[abc] 6
Public Sub Main()
    Dim s As String
    s = LeftB$("abc", 2)
    Debug.Print "a=[" & s & "] " & CStr(LenB(s))
    s = LeftB$("abc", 4)
    Debug.Print "b=[" & s & "] " & CStr(LenB(s))
    s = LeftB$("abc", 100)
    Debug.Print "c=[" & s & "] " & CStr(LenB(s))
End Sub
