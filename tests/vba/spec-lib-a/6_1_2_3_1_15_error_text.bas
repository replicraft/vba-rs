' INV: vbal.6.1.2.3.1.15, func.error
' SOURCE: spec ms-vbal 6.1.2.3.1.15 Error/Error$ descriptive texts
' EXPECT: zero=[]
' EXPECT: zeroDollar=[]
' EXPECT: e5=Invalid procedure call or argument
' EXPECT: e5s=Invalid procedure call or argument
' EXPECT: e6=Overflow
' EXPECT: e13=Type mismatch
' EXPECT: e94=Invalid use of Null
' EXPECT: str=Type mismatch
' EXPECT: frac=Overflow
' EXPECT: type=String
' EXPECT: typeVar=String
Public Sub Main()
    Debug.Print "zero=[" & Error(0) & "]"
    Debug.Print "zeroDollar=[" & Error$(0) & "]"
    Debug.Print "e5=" & Error(5)
    Debug.Print "e5s=" & Error$(5)
    Debug.Print "e6=" & Error$(6)
    Debug.Print "e13=" & Error$(13)
    Debug.Print "e94=" & Error$(94)
    Debug.Print "str=" & Error$("13")
    Debug.Print "frac=" & Error$(6.4)
    Debug.Print "type=" & TypeName(Error$(5))
    Debug.Print "typeVar=" & TypeName(Error(5))
End Sub
