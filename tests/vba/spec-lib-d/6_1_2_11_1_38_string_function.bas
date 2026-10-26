' INV: vbal.6.1.2.11.1.38, func.string
' SOURCE: spec ms-vbal 6.1.2.11.1.38 String repeats first character; numeric code Mod 256
' EXPECT: a=aaa
' EXPECT: b=xxxx
' EXPECT: c=AAA
' EXPECT: d=AA
' EXPECT: g=[]
' EXPECT: t=String
' EXPECT: h=zzz
Public Sub Main()
    Debug.Print "a=" & String(3, "a")
    Debug.Print "b=" & String(4, "xyz")
    Debug.Print "c=" & String(3, 65)
    Debug.Print "d=" & String(2, 321)
    Debug.Print "g=[" & String(0, "a") & "]"
    Debug.Print "t=" & TypeName(String(2, "x"))
    Debug.Print "h=" & String$(3, "z")
End Sub
