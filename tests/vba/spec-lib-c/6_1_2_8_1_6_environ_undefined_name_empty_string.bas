' INV: vbal.6.1.2.8.1.6, func.environ
' SOURCE: spec ms-vbal 6.1.2.8.1.6 Environ of an undefined name is a zero-length string
' EXPECT: len=0
' EXPECT: len$=0
' EXPECT: String
' EXPECT: String
Public Sub Main()
    Debug.Print "len=" & Len(Environ("VBARS_SURELY_UNDEFINED_VARIABLE_XYZ"))
    Debug.Print "len$=" & Len(Environ$("VBARS_SURELY_UNDEFINED_VARIABLE_XYZ"))
    Debug.Print TypeName(Environ("VBARS_SURELY_UNDEFINED_VARIABLE_XYZ"))
    Debug.Print TypeName(Environ$("VBARS_SURELY_UNDEFINED_VARIABLE_XYZ"))
End Sub
