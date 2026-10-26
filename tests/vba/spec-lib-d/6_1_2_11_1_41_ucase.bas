' INV: vbal.6.1.2.11.1.41, func.ucase
' SOURCE: derived ms-vbal 6.1.2.11.1.41 UCase upper-cases letters and leaves other characters alone
' EXPECT: a=ABC-XYZ 12
' EXPECT: e=[]
' EXPECT: n=True
Public Sub Main()
    Debug.Print "a=" & UCase("AbC-xyz 12")
    Debug.Print "e=[" & UCase("") & "]"
    Debug.Print "n=" & CStr(IsNull(UCase(Null)))
End Sub
