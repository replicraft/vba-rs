' INV: vbal.6.1.2.11.1.27
' SOURCE: derived ms-vbal 6.1.2.11.1.27 MidB$ counts bytes from a 1-based start; a character takes two bytes
' EXPECT: a=[b]
' EXPECT: b=[bc]
' EXPECT: c=[]
Public Sub Main()
    Debug.Print "a=[" & MidB$("abc", 3, 2) & "]"
    Debug.Print "b=[" & MidB$("abc", 3) & "]"
    Debug.Print "c=[" & MidB$("abc", 7) & "]"
End Sub
