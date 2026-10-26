' INV: vbal.6.1.2.11.1.42, func.ucase
' SOURCE: derived ms-vbal 6.1.2.11.1.42 UCase$ upper-cases like UCase and returns a String
' EXPECT: a=ABC-XYZ 12
' EXPECT: t=String
Public Sub Main()
    Debug.Print "a=" & UCase$("AbC-xyz 12")
    Debug.Print "t=" & TypeName(UCase$("a"))
End Sub
