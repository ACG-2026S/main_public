Shader "Custom/Shadow0"
{
  Properties
  {
    _LightPos ("Light Position", Vector) = (0,3,0)
  }

  SubShader
  {
    Tags { "RenderType" = "Opaque" "RenderPipeline" = "UniversalPipeline" }
    Pass {
      HLSLPROGRAM
      #pragma vertex vert
      #pragma fragment frag
      #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

      struct Attributes {
        float4 positionOS : POSITION;
        float4 normalOS : NORMAL;
      };

      struct Varyings {
        float4 positionHCS : SV_POSITION;
        float3 normalWS : TEXCOORD0;
      };

      float4 _LightPos;

      Varyings vert(Attributes IN) {
        Varyings OUT;
        OUT.positionHCS = TransformObjectToHClip(IN.positionOS.xyz);
        OUT.normalWS = TransformObjectToWorldNormal(IN.normalOS.xyz); 
        return OUT;
      } 

      half4 frag(Varyings IN) : SV_Target {
        half4 color = half4(IN.normalWS * 0.5 + 0.5, 1.0);
        return color;
      }
      ENDHLSL
    }
  }
}

