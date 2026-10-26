' INV: vbal.6.1.2.11.1.18
' SOURCE: derived ms-vbal 6.1.2.11.1.18 LCase$ lower-cases like LCase and returns a String
' EXPECT: a=abc-xyz 12
' EXPECT: t=String
' EXPECT: e=[]
Public Sub Main()
    Debug.Print "a=" & LCase$("AbC-XYZ 12")
    Debug.Print "t=" & TypeName(LCase$("A"))
    Debug.Print "e=[" & LCase$("") & "]"
End Sub
