' INV: vbal.3.3.1, lex.colon
' SOURCE: spec ms-vbal 3.3.1 separator tokens, EOS is any run of colons and line ends
' EXPECT: ab=12
' EXPECT: ab=34
' EXPECT: ab=56
Public Sub Main()
    Dim a As Long, b As Long
    a = 1: b = 2
    Debug.Print "ab=" & a & b
    a = 3 :: b = 4
    Debug.Print "ab=" & a & b
    : :
    a = 5 : : b = 6 :
    Debug.Print "ab=" & a & b: ' trailing comment after a colon
End Sub
